const $ = (id) => document.getElementById(id);

export function bindLevelSelect(onSelect) {
    $('level-select').addEventListener('click', (e) => {
        if (e.target.tagName === 'LI') {
            document.querySelectorAll('#level-select li').forEach(li => li.classList.remove('active'));
            e.target.classList.add('active');
            onSelect(e.target.dataset.level);
        }
    });
}
