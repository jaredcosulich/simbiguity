const { test } = require('node:test');
const assert = require('node:assert');
const fs = require('fs');
const path = require('path');

function walkImports(baseDir, startFile, visited = new Set()) {
    const filePath = path.resolve(baseDir, startFile);
    if (visited.has(filePath)) return;
    visited.add(filePath);

    if (!fs.existsSync(filePath)) {
        throw new Error(`Missing file: ${filePath} (imported from ${baseDir})`);
    }

    const content = fs.readFileSync(filePath, 'utf-8');
    
    if (filePath.endsWith('.html')) {
        const scriptRe = /<script[^>]+src=["']([^"']+)["']/g;
        let match;
        while ((match = scriptRe.exec(content)) !== null) {
            walkImports(path.dirname(filePath), match[1], visited);
        }
    } else if (filePath.endsWith('.js') || filePath.endsWith('.mjs')) {
        const importRe = /from\s+["']([^"']+)["']/g;
        let match;
        while ((match = importRe.exec(content)) !== null) {
            walkImports(path.dirname(filePath), match[1], visited);
        }
        
        const dynamicImportRe = /import\s*\(\s*new\s*URL\s*\(\s*["']([^"']+)["']/g;
// Every demo directory is in the manifest
        let dynMatch;
        while ((dynMatch = dynamicImportRe.exec(content)) !== null) {
            walkImports(path.dirname(filePath), dynMatch[1], visited);
        }
    }
}

test('every_demo_directory_is_in_the_manifest', () => {
    const demosDir = path.join(__dirname, 'demos');
    const manifestPath = path.join(demosDir, 'manifest.json');
    assert.ok(fs.existsSync(manifestPath), 'manifest.json exists');
    
    const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf-8'));
    const manifestSlugs = new Set(manifest.map(m => m.slug));
    
// Smoke test all demos resolve their imports
    const dirs = fs.readdirSync(demosDir, { withFileTypes: true })
        .filter(d => d.isDirectory())
        .map(d => d.name);
        
    for (const dir of dirs) {
        assert.ok(manifestSlugs.has(dir), `Demo directory ${dir} is not in manifest.json`);
    }
});

test('smoke test all demos resolve their imports', () => {
    const demosDir = path.join(__dirname, 'demos');
    const dirs = fs.readdirSync(demosDir, { withFileTypes: true })
        .filter(d => d.isDirectory())
        .map(d => d.name);
        
    for (const dir of dirs) {
        const indexHtml = path.join(demosDir, dir, 'index.html');
        if (fs.existsSync(indexHtml)) {
            walkImports(path.dirname(indexHtml), 'index.html');
        }
    }
});
