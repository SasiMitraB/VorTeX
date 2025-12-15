import { state } from './state.js';

export async function updateOutline(filePath) {
    const outlineTree = document.getElementById('outline-tree');
    if (!outlineTree) return;

    outlineTree.innerHTML = '';

    if (!filePath || !filePath.endsWith('.tex')) {
        outlineTree.innerHTML = '<div style="padding: 8px; color: var(--muted); font-size: 12px;">No outline available</div>';
        return;
    }

    try {
        const sections = await window.latexServices.getSections(filePath);

        if (!sections || sections.length === 0) {
            outlineTree.innerHTML = '<div style="padding: 8px; color: var(--muted); font-size: 12px;">No sections found</div>';
            return;
        }

        sections.forEach(section => {
            const el = document.createElement('div');
            el.className = `outline-node level-${section.level}`;
            el.innerText = section.title;
            el.title = section.title;
            el.onclick = () => {
                // Navigate to section
                const activeEditor = state.editors[state.activePane];
                if (activeEditor) {
                    activeEditor.revealLineInCenter(section.line);
                    activeEditor.setPosition({ lineNumber: section.line, column: 1 });
                    activeEditor.focus();
                }
            };
            outlineTree.appendChild(el);
        });
    } catch (error) {
        console.error('Error updating outline:', error);
    }
}
