/**
 * Stratum Debug Adapter
 *
 * This script spawns the Stratum DAP server and acts as a bridge between
 * VS Code and the native debug adapter.
 */

import * as child_process from 'child_process';
import * as net from 'net';
import * as path from 'path';

// Get the stratum executable path from environment or default
const stratumPath = process.env.STRATUM_PATH || 'stratum';

// Spawn the Stratum DAP server
const dapProcess = child_process.spawn(stratumPath, ['dap'], {
    stdio: ['pipe', 'pipe', 'pipe'],
    env: process.env,
});

// Forward stdin to the DAP process
process.stdin.pipe(dapProcess.stdin);

// Forward DAP process stdout to our stdout
dapProcess.stdout.pipe(process.stdout);

// Forward DAP process stderr to our stderr
dapProcess.stderr.pipe(process.stderr);

// Handle process exit
dapProcess.on('exit', (code) => {
    process.exit(code ?? 0);
});

dapProcess.on('error', (err) => {
    console.error(`Failed to start Stratum DAP server: ${err.message}`);
    process.exit(1);
});

// Handle our own termination
process.on('SIGTERM', () => {
    dapProcess.kill('SIGTERM');
});

process.on('SIGINT', () => {
    dapProcess.kill('SIGINT');
});
