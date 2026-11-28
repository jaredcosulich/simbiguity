export function bindPartsTray(onSelectKind) {
    document.querySelectorAll('.tray-item').forEach(item => {
        item.onclick = (e) => {
            document.querySelectorAll('.tray-item').forEach(i => i.classList.remove('selected'));
            e.currentTarget.classList.add('selected');
            const part = e.currentTarget.dataset.part;
            let selectedKind = 4; // default wire
            if (part === 'wire') selectedKind = 4;
            if (part === 'bulb') selectedKind = 3;
            if (part === 'switch') selectedKind = 2;
            if (part === 'battery') selectedKind = 1;
            onSelectKind(selectedKind);
        };
    });
}