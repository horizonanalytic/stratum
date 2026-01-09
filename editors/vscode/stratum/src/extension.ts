import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
    Trace,
} from 'vscode-languageclient/node';
import { StratumTaskProvider, createManifestWatcher } from './taskProvider';

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
    const outputChannel = vscode.window.createOutputChannel('Stratum');
    outputChannel.appendLine('Stratum extension is activating...');

    // Get configuration
    const config = vscode.workspace.getConfiguration('stratum');
    const serverPath = config.get<string>('server.path', 'stratum');
    const serverArgs = config.get<string[]>('server.args', ['lsp']);
    const trace = config.get<string>('trace.server', 'off');

    // Server options: run the stratum lsp command
    const serverOptions: ServerOptions = {
        run: {
            command: serverPath,
            args: serverArgs,
            transport: TransportKind.stdio,
        },
        debug: {
            command: serverPath,
            args: serverArgs,
            transport: TransportKind.stdio,
        },
    };

    // Client options
    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'stratum' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.strat'),
        },
        outputChannel,
        traceOutputChannel: outputChannel,
    };

    // Create and start the language client
    client = new LanguageClient(
        'stratum',
        'Stratum Language Server',
        serverOptions,
        clientOptions
    );

    // Set trace level
    if (trace !== 'off') {
        client.setTrace(trace === 'verbose'
            ? Trace.Verbose
            : Trace.Messages);
    }

    // Register restart command
    const restartCommand = vscode.commands.registerCommand(
        'stratum.restartServer',
        async () => {
            outputChannel.appendLine('Restarting Stratum language server...');
            if (client) {
                await client.stop();
                await client.start();
                outputChannel.appendLine('Stratum language server restarted.');
            }
        }
    );
    context.subscriptions.push(restartCommand);

    // Register format on save if enabled
    const formatOnSave = config.get<boolean>('format.onSave', true);
    if (formatOnSave) {
        const formatDisposable = vscode.workspace.onWillSaveTextDocument(
            async (event) => {
                if (event.document.languageId === 'stratum') {
                    const formatEdits = vscode.commands.executeCommand<vscode.TextEdit[]>(
                        'vscode.executeFormatDocumentProvider',
                        event.document.uri
                    );
                    event.waitUntil(formatEdits.then(edits => edits || []));
                }
            }
        );
        context.subscriptions.push(formatDisposable);
    }

    // Start the client
    try {
        await client.start();
        outputChannel.appendLine('Stratum language server started successfully.');
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        outputChannel.appendLine(`Failed to start Stratum language server: ${message}`);
        vscode.window.showErrorMessage(
            `Failed to start Stratum language server: ${message}. ` +
            `Make sure 'stratum' is installed and in your PATH.`
        );
    }

    context.subscriptions.push(client);

    // Register debug adapter factory
    const debugOutputChannel = vscode.window.createOutputChannel('Stratum Debug');
    const debugAdapterFactory = new StratumDebugAdapterFactory(serverPath, debugOutputChannel);
    context.subscriptions.push(
        vscode.debug.registerDebugAdapterDescriptorFactory('stratum', debugAdapterFactory)
    );
    context.subscriptions.push(debugAdapterFactory);

    // Register debug configuration provider
    const configProvider = new StratumDebugConfigurationProvider();
    context.subscriptions.push(
        vscode.debug.registerDebugConfigurationProvider('stratum', configProvider)
    );

    outputChannel.appendLine('Stratum debug adapter registered.');

    // Register task provider
    const taskProvider = new StratumTaskProvider(serverPath);
    context.subscriptions.push(
        vscode.tasks.registerTaskProvider(StratumTaskProvider.StratumType, taskProvider)
    );

    // Watch for stratum.toml changes to refresh tasks
    const manifestWatcher = createManifestWatcher(taskProvider);
    context.subscriptions.push(manifestWatcher);

    outputChannel.appendLine('Stratum task provider registered.');
}

export async function deactivate(): Promise<void> {
    if (client) {
        await client.stop();
    }
}

/**
 * Debug adapter factory that spawns the Stratum DAP server
 */
class StratumDebugAdapterFactory implements vscode.DebugAdapterDescriptorFactory {
    private stratumPath: string;
    private outputChannel: vscode.OutputChannel;

    constructor(stratumPath: string, outputChannel: vscode.OutputChannel) {
        this.stratumPath = stratumPath;
        this.outputChannel = outputChannel;
    }

    createDebugAdapterDescriptor(
        _session: vscode.DebugSession,
        _executable: vscode.DebugAdapterExecutable | undefined
    ): vscode.ProviderResult<vscode.DebugAdapterDescriptor> {
        this.outputChannel.appendLine(`Starting Stratum debug adapter: ${this.stratumPath} dap`);

        // Spawn the Stratum DAP server as an executable
        return new vscode.DebugAdapterExecutable(this.stratumPath, ['dap']);
    }

    dispose(): void {
        // Nothing to dispose
    }
}

/**
 * Debug configuration provider for Stratum
 */
class StratumDebugConfigurationProvider implements vscode.DebugConfigurationProvider {
    resolveDebugConfiguration(
        folder: vscode.WorkspaceFolder | undefined,
        config: vscode.DebugConfiguration,
        _token?: vscode.CancellationToken
    ): vscode.ProviderResult<vscode.DebugConfiguration> {
        // If no configuration is provided, create a default one
        if (!config.type && !config.request && !config.name) {
            const editor = vscode.window.activeTextEditor;
            if (editor && editor.document.languageId === 'stratum') {
                config.type = 'stratum';
                config.name = 'Debug Stratum File';
                config.request = 'launch';
                config.program = '${file}';
                config.stopOnEntry = false;
            }
        }

        // Ensure program is specified
        if (!config.program) {
            return vscode.window.showInformationMessage('Cannot find a program to debug').then(() => {
                return undefined;
            });
        }

        // Resolve ${file} and similar variables
        if (config.program) {
            config.program = this.resolveVariables(config.program, folder);
        }

        return config;
    }

    private resolveVariables(value: string, folder: vscode.WorkspaceFolder | undefined): string {
        const editor = vscode.window.activeTextEditor;

        // Replace ${file}
        if (editor) {
            value = value.replace(/\${file}/g, editor.document.uri.fsPath);
        }

        // Replace ${workspaceFolder}
        if (folder) {
            value = value.replace(/\${workspaceFolder}/g, folder.uri.fsPath);
        }

        return value;
    }
}
