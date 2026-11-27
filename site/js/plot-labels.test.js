import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';
import { renderLabels } from './plot-labels.js';

describe('renderLabels', () => {
    let container;

    beforeEach(() => {
        global.document = {
            createElement: (tag) => {
                return {
                    tag,
                    className: "",
                    children: [],
                    style: {},
                    innerHTML: "",
                    appendChild(child) { this.children.push(child); },
                    querySelector(selector) {
                        if (selector.startsWith(".")) {
                            return this.children.find(c => c.className === selector.substring(1)) || null;
                        }
                        return this.children.find(c => c.tag === selector) || null;
                    },
                    querySelectorAll(selector) {
                        if (selector.startsWith(".")) {
                            return this.children.filter(c => c.className === selector.substring(1));
                        }
                        return this.children.filter(c => c.tag === selector);
                    }
                };
            },
            body: {
                children: [],
                appendChild(child) { this.children.push(child); }
            }
        };
        container = document.createElement('div');
        document.body.appendChild(container);
    });

    it('renders labels into a dedicated overlay without clearing siblings', () => {
        const sibling = document.createElement('canvas');
        container.appendChild(sibling);

        renderLabels(container, [
            ["1.0", 10.5, 20.0],
            ["0", 50, 60],
        ]);

        assert.strictEqual(container.children[0], sibling, "Sibling canvas should survive");
        assert.strictEqual(container.children.length, 2, "Should have sibling and overlay");
        
        const overlay = container.children[1];
        assert.strictEqual(overlay.className, "plot-labels-overlay");
        assert.strictEqual(overlay.children.length, 2);

        const spans = overlay.children;
        assert.strictEqual(spans[0].textContent, "1.0");
        assert.strictEqual(spans[0].style.position, "absolute");
        assert.strictEqual(spans[0].style.left, "10.5px");
        assert.strictEqual(spans[0].style.top, "20px");
    });
});
