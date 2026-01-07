import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
    Trace,
} from 'vscode-languageclient/node';

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
}

export async function deactivate(): Promise<void> {
    if (client) {
        await client.stop();
    }
}
