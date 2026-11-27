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
                    children: [],
                    style: {},
                    innerHTML: "",
                    appendChild(child) { this.children.push(child); },
                    querySelectorAll(selector) { return this.children.filter(c => c.tag === selector); }
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

    it('renders labels as spans with absolute positioning', () => {
        renderLabels(container, [
            ["1.0", 10.5, 20.0],
            ["0", 50, 60],
        ]);

        const spans = container.querySelectorAll('span');
        assert.strictEqual(spans.length, 2);

        assert.strictEqual(spans[0].textContent, "1.0");
        assert.strictEqual(spans[0].style.position, "absolute");
        assert.strictEqual(spans[0].style.left, "10.5px");
        assert.strictEqual(spans[0].style.top, "20px");

        assert.strictEqual(spans[1].textContent, "0");
        assert.strictEqual(spans[1].style.left, "50px");
        assert.strictEqual(spans[1].style.top, "60px");
    });
});
