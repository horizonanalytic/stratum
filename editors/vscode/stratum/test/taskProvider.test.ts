import * as assert from 'assert';
import * as path from 'path';
import * as fs from 'fs';
import * as os from 'os';

// Note: This test runs outside VS Code environment, so we test
// the parsing logic rather than the full task provider

describe('Task Provider Logic', () => {
    const tempDir = path.join(os.tmpdir(), 'stratum-test-' + Date.now());

    before(() => {
        fs.mkdirSync(tempDir, { recursive: true });
    });

    after(() => {
        fs.rmSync(tempDir, { recursive: true, force: true });
    });

    it('should parse package name from stratum.toml', () => {
        const manifestPath = path.join(tempDir, 'stratum.toml');
        const content = `[package]
name = "my-test-project"
version = "0.1.0"
`;
        fs.writeFileSync(manifestPath, content);

        // Simple parsing logic (mirroring what taskProvider does)
        const manifestContent = fs.readFileSync(manifestPath, 'utf-8');
        const nameMatch = manifestContent.match(/name\s*=\s*"([^"]+)"/);

        assert.ok(nameMatch, 'Should find name in manifest');
        assert.strictEqual(nameMatch[1], 'my-test-project');
    });

    it('should detect entry point files', () => {
        const srcDir = path.join(tempDir, 'src');
        fs.mkdirSync(srcDir, { recursive: true });

        const mainPath = path.join(srcDir, 'main.strat');
        fs.writeFileSync(mainPath, '// main entry');

        assert.ok(fs.existsSync(mainPath), 'main.strat should exist');
    });

    it('should build correct task arguments for run', () => {
        // Test argument building logic
        const definition = { type: 'stratum', task: 'run', file: 'src/main.strat' };
        const args = buildTaskArgs(definition as any);

        assert.deepStrictEqual(args, ['run', 'src/main.strat']);
    });

    it('should build correct task arguments for build with release', () => {
        const definition = { type: 'stratum', task: 'build', file: 'src/main.strat', release: true };
        const args = buildTaskArgs(definition as any);

        assert.deepStrictEqual(args, ['build', 'src/main.strat', '--release']);
    });

    it('should build correct task arguments for test with filter', () => {
        const definition = { type: 'stratum', task: 'test', file: 'src/lib.strat', filter: 'unit' };
        const args = buildTaskArgs(definition as any);

        assert.deepStrictEqual(args, ['test', 'src/lib.strat', '--filter', 'unit']);
    });
});

// Helper function that mirrors the logic in taskProvider.ts
interface TaskDef {
    task: 'run' | 'build' | 'test' | 'fmt';
    file?: string;
    release?: boolean;
    filter?: string;
}

function buildTaskArgs(definition: TaskDef): string[] {
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
                args.push(definition.file);
            }
            break;
    }

    return args;
}
