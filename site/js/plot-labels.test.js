import { describe, it, expect, beforeEach } from 'vitest';
import { renderLabels } from './plot-labels.js';

describe('renderLabels', () => {
    let container;

    beforeEach(() => {
        container = document.createElement('div');
        document.body.appendChild(container);
    });

    it('renders labels as spans with absolute positioning', () => {
        renderLabels(container, [
            ["1.0", 10.5, 20.0],
            ["0", 50, 60],
        ]);

        const spans = container.querySelectorAll('span');
        expect(spans.length).toBe(2);

        expect(spans[0].textContent).toBe("1.0");
        expect(spans[0].style.position).toBe("absolute");
        expect(spans[0].style.left).toBe("10.5px");
        expect(spans[0].style.top).toBe("20px");

        expect(spans[1].textContent).toBe("0");
        expect(spans[1].style.left).toBe("50px");
        expect(spans[1].style.top).toBe("60px");
    });
});
