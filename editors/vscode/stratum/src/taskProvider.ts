/**
 * Task provider for Stratum projects.
 *
 * Automatically detects stratum.toml projects and provides build/run/test tasks.
 */

import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

/** Task definition for Stratum tasks */
interface StratumTaskDefinition extends vscode.TaskDefinition {
    /** The task type: run, build, test, fmt */
    task: 'run' | 'build' | 'test' | 'fmt';
    /** The file to operate on */
    file?: string;
    /** Build with optimizations (for build task) */
    release?: boolean;
    /** Test filter pattern (for test task) */
    filter?: string;
}

/** Cached project information */
interface StratumProject {
    /** Path to stratum.toml */
    manifestPath: string;
    /** Project root directory */
    rootDir: string;
    /** Main entry point (src/main.strat or src/lib.strat) */
    entryPoint?: string;
    /** Project name from manifest */
    name?: string;
}

/**
 * Task provider for Stratum projects.
 *
 * Automatically detects stratum.toml files in the workspace and provides
 * common tasks: run, build, test, and format.
 */
export class StratumTaskProvider implements vscode.TaskProvider {
    static readonly StratumType = 'stratum';
    private stratumPath: string;
    private projectCache: Map<string, StratumProject> = new Map();

    constructor(stratumPath: string) {
        this.stratumPath = stratumPath;
    }

    /**
     * Provide all available tasks for Stratum projects in the workspace.
     */
    async provideTasks(): Promise<vscode.Task[]> {
        const tasks: vscode.Task[] = [];
        const workspaceFolders = vscode.workspace.workspaceFolders;

        if (!workspaceFolders) {
            return tasks;
        }

        // Find all stratum.toml files in the workspace
        for (const folder of workspaceFolders) {
            const projects = await this.findStratumProjects(folder.uri.fsPath);

            for (const project of projects) {
                // Create tasks for each project
                tasks.push(...this.createProjectTasks(project, folder));
            }
        }

        return tasks;
    }

    /**
     * Resolve a task that was loaded from tasks.json.
     */
    resolveTask(task: vscode.Task): vscode.Task | undefined {
        const definition = task.definition as StratumTaskDefinition;

        if (definition.task) {
            return this.createTask(
                definition,
                task.scope ?? vscode.TaskScope.Workspace,
                task.name
            );
        }

        return undefined;
    }

    /**
     * Find all Stratum projects in a directory.
     */
    private async findStratumProjects(rootPath: string): Promise<StratumProject[]> {
        const projects: StratumProject[] = [];

        // Check for stratum.toml in root
        const rootManifest = path.join(rootPath, 'stratum.toml');
        if (fs.existsSync(rootManifest)) {
            const project = await this.parseProject(rootManifest);
            if (project) {
                projects.push(project);
            }
        }

        // Also check immediate subdirectories for workspace-like setups
        try {
            const entries = fs.readdirSync(rootPath, { withFileTypes: true });
            for (const entry of entries) {
                if (entry.isDirectory() && !entry.name.startsWith('.')) {
                    const subManifest = path.join(rootPath, entry.name, 'stratum.toml');
                    if (fs.existsSync(subManifest)) {
                        const project = await this.parseProject(subManifest);
                        if (project) {
                            projects.push(project);
                        }
                    }
                }
            }
        } catch {
            // Ignore errors reading directory
        }

        return projects;
    }

    /**
     * Parse a stratum.toml manifest and extract project information.
     */
    private async parseProject(manifestPath: string): Promise<StratumProject | null> {
        // Check cache first
        if (this.projectCache.has(manifestPath)) {
            return this.projectCache.get(manifestPath)!;
        }

        try {
            const rootDir = path.dirname(manifestPath);
            const content = fs.readFileSync(manifestPath, 'utf-8');

            // Simple TOML parsing for name
            const nameMatch = content.match(/name\s*=\s*"([^"]+)"/);
            const name = nameMatch ? nameMatch[1] : path.basename(rootDir);

            // Determine entry point
            const mainPath = path.join(rootDir, 'src', 'main.strat');
            const libPath = path.join(rootDir, 'src', 'lib.strat');

            let entryPoint: string | undefined;
            if (fs.existsSync(mainPath)) {
                entryPoint = mainPath;
            } else if (fs.existsSync(libPath)) {
                entryPoint = libPath;
            }

            const project: StratumProject = {
                manifestPath,
                rootDir,
                entryPoint,
                name,
            };

            this.projectCache.set(manifestPath, project);
            return project;
        } catch {
            return null;
        }
    }

    /**
     * Create all standard tasks for a Stratum project.
     */
    private createProjectTasks(
        project: StratumProject,
        folder: vscode.WorkspaceFolder
    ): vscode.Task[] {
        const tasks: vscode.Task[] = [];
        const projectLabel = project.name || path.basename(project.rootDir);

        // Run task (if main.strat exists)
        if (project.entryPoint?.endsWith('main.strat')) {
            tasks.push(
                this.createTask(
                    { type: StratumTaskProvider.StratumType, task: 'run', file: project.entryPoint },
                    folder,
                    `Run ${projectLabel}`
                )
            );
        }

        // Build task (if main.strat exists)
        if (project.entryPoint?.endsWith('main.strat')) {
            tasks.push(
                this.createTask(
                    { type: StratumTaskProvider.StratumType, task: 'build', file: project.entryPoint },
                    folder,
                    `Build ${projectLabel}`
                )
            );

            // Release build
            tasks.push(
                this.createTask(
                    { type: StratumTaskProvider.StratumType, task: 'build', file: project.entryPoint, release: true },
                    folder,
                    `Build ${projectLabel} (Release)`
                )
            );
        }

        // Test task
        const testFile = project.entryPoint || path.join(project.rootDir, 'src', 'lib.strat');
        if (fs.existsSync(testFile)) {
            tasks.push(
                this.createTask(
                    { type: StratumTaskProvider.StratumType, task: 'test', file: testFile },
                    folder,
                    `Test ${projectLabel}`
                )
            );
        }

        // Format task
        tasks.push(
            this.createTask(
                { type: StratumTaskProvider.StratumType, task: 'fmt', file: project.rootDir },
                folder,
                `Format ${projectLabel}`
            )
        );

        return tasks;
    }

    /**
     * Create a single task with the appropriate command and problem matcher.
     */
    private createTask(
        definition: StratumTaskDefinition,
        scope: vscode.TaskScope | vscode.WorkspaceFolder,
        name: string
    ): vscode.Task {
        // Build the command arguments
        const args = this.buildTaskArgs(definition);

        // Create shell execution
        const execution = new vscode.ShellExecution(this.stratumPath, args);

        // Determine task group
        let group: vscode.TaskGroup | undefined;
        if (definition.task === 'build') {
            group = vscode.TaskGroup.Build;
        } else if (definition.task === 'test') {
            group = vscode.TaskGroup.Test;
        }

        // Create the task
        const task = new vscode.Task(
            definition,
            scope,
            name,
            'stratum',
            execution,
            ['$stratum', '$stratum-simple']
        );

        task.group = group;
        task.presentationOptions = {
            reveal: vscode.TaskRevealKind.Always,
            panel: vscode.TaskPanelKind.Shared,
            clear: true,
        };

        return task;
    }

    /**
     * Build command line arguments for a task.
     */
    private buildTaskArgs(definition: StratumTaskDefinition): string[] {
        const args: string[] = [definition.task];

        switch (definition.task) {
            case 'run':
                if (definition.file) {
                    args.push(definition.file);
                }
                break;

            case 'build':
                if (definition.file) {
                    args.push(definition.file);
                }
                if (definition.release) {
                    args.push('--release');
                }
                break;

            case 'test':
                if (definition.file) {
                    args.push(definition.file);
                }
                if (definition.filter) {
                    args.push('--filter', definition.filter);
                }
                break;

            case 'fmt':
                if (definition.file) {
                    // For format, we pass the directory or file
                    args.push(definition.file);
                }
                break;
        }

        return args;
    }

    /**
     * Clear the project cache (useful when stratum.toml files change).
     */
    clearCache(): void {
        this.projectCache.clear();
    }
}

/**
 * Create a file system watcher for stratum.toml files.
 *
 * When a stratum.toml is created, modified, or deleted, the task provider
 * cache is cleared so tasks are regenerated.
 */
export function createManifestWatcher(
    taskProvider: StratumTaskProvider
): vscode.FileSystemWatcher {
    const watcher = vscode.workspace.createFileSystemWatcher('**/stratum.toml');

    watcher.onDidCreate(() => taskProvider.clearCache());
    watcher.onDidChange(() => taskProvider.clearCache());
    watcher.onDidDelete(() => taskProvider.clearCache());

    return watcher;
}
