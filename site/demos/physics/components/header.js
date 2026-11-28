export function PhysicsHeader() {
    const header = document.createElement('header');
    header.innerHTML = `
        <h1>Physics Lab</h1>
        <a href="/">Back to home</a>
    `;
    return header;
}
