import * as assert from 'assert';
import * as fs from 'fs';
import * as path from 'path';

describe('Stratum TextMate Grammar', () => {
    const grammarPath = path.join(__dirname, '..', '..', 'syntaxes', 'stratum.tmLanguage.json');
    let grammar: Record<string, unknown>;

    before(() => {
        const content = fs.readFileSync(grammarPath, 'utf8');
        grammar = JSON.parse(content);
    });

    it('should be valid JSON', () => {
        assert.ok(grammar, 'Grammar should parse as JSON');
    });

    it('should have correct scope name', () => {
        assert.strictEqual(grammar.scopeName, 'source.stratum');
    });

    it('should have patterns defined', () => {
        assert.ok(Array.isArray(grammar.patterns), 'Should have patterns array');
        assert.ok((grammar.patterns as unknown[]).length > 0, 'Should have at least one pattern');
    });

    it('should have repository with required sections', () => {
        const repo = grammar.repository as Record<string, unknown>;
        assert.ok(repo, 'Should have repository');

        const requiredSections = [
            'comments',
            'strings',
            'numbers',
            'keywords',
            'operators',
            'function-definition',
            'struct-definition',
            'enum-definition'
        ];

        for (const section of requiredSections) {
            assert.ok(repo[section], `Should have '${section}' in repository`);
        }
    });

    it('should define all Stratum keywords', () => {
        const repo = grammar.repository as Record<string, { patterns: Array<{ match: string }> }>;
        const keywordPatterns = repo.keywords?.patterns || [];
        const allPatterns = keywordPatterns.map(p => p.match).join(' ');

        const expectedKeywords = [
            'fx', 'let', 'if', 'else', 'for', 'while', 'match',
            'return', 'break', 'continue', 'in', 'struct', 'enum',
            'interface', 'impl', 'async', 'await', 'try', 'catch',
            'throw', 'import', 'true', 'false', 'null'
        ];

        for (const keyword of expectedKeywords) {
            assert.ok(
                allPatterns.includes(keyword),
                `Should define keyword '${keyword}'`
            );
        }
    });
});
