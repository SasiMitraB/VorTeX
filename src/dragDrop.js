import { state, setActivePane } from './state.js';
import { switchTab } from './tabs.js';
import { hideDropZones, setPdfPointerEvents } from './dom.js';

export function setupDropZones() {
  const dropZones = document.querySelectorAll('.drop-zone');

  dropZones.forEach(zone => {
    zone.addEventListener('dragenter', (e) => {
      if (!state.draggedTab) return;
      e.preventDefault();
      e.stopPropagation();
      zone.classList.add('active');
    });

    zone.addEventListener('dragleave', (e) => {
      if (!state.draggedTab) return;
      e.preventDefault();
      e.stopPropagation();
      zone.classList.remove('active');
    });

    zone.addEventListener('dragover', (e) => {
      if (!state.draggedTab) return;
      e.preventDefault();
      e.stopPropagation();
    });

    zone.addEventListener('drop', (e) => {
      if (!state.draggedTab) return;
      e.preventDefault();
      e.stopPropagation();

      const targetPane = zone.dataset.target;
      zone.classList.remove('active');

      let finalTargetPane = targetPane;
      if (targetPane === 'left' && state.draggedTab.fromPane === 'left') {
        const rightPane = document.getElementById('editor-pane-right');
        if (rightPane.style.display === 'none') {
          finalTargetPane = 'right';
        }
      }

      if (state.draggedTab.fromPane !== finalTargetPane) {
        const fromPane = state.draggedTab.fromPane;
        const tab = state.draggedTab.tab;

        const idx = state.paneTabs[fromPane].findIndex(t => t.id === tab.id);
        if (idx !== -1) {
          state.paneTabs[fromPane].splice(idx, 1);
          if (state.paneState[fromPane].activeId === tab.id) {
            if (state.paneTabs[fromPane].length > 0) {
              switchTab(state.paneTabs[fromPane][0].id, fromPane);
            } else {
              state.paneState[fromPane].activeId = null;
            }
          }
        }

        state.paneTabs[finalTargetPane].push(tab);
        if (finalTargetPane === 'right') {
          document.getElementById('editor-pane-right').style.display = 'flex';
        }
        switchTab(tab.id, finalTargetPane);
        setActivePane(finalTargetPane);
      }
      hideDropZones();
      setPdfPointerEvents(true);
    });
  });
}

export function setupGlobalDragHandlers() {
  document.addEventListener('dragover', (e) => {
    if (!state.draggedTab) {
      hideDropZones();
      setPdfPointerEvents(true);
      return;
    }

    const leftPane = document.getElementById('editor-pane-left');
    const rightPane = document.getElementById('editor-pane-right');
    const leftRect = leftPane.getBoundingClientRect();
    const rightRect = rightPane.getBoundingClientRect();

    const leftDropZone = leftPane.querySelector('.drop-zone');
    const rightDropZone = rightPane.querySelector('.drop-zone');

    const midLeftPane = leftRect.left + leftRect.width / 2;

    if (state.draggedTab.fromPane === 'right' &&
        e.clientX >= midLeftPane && e.clientX <= leftRect.right &&
        e.clientY >= leftRect.top && e.clientY <= leftRect.bottom) {
      leftDropZone.style.display = 'flex';
      leftDropZone.style.pointerEvents = 'auto';
      leftDropZone.classList.add('active');
    } else {
      leftDropZone.style.display = 'none';
      leftDropZone.style.pointerEvents = 'none';
      leftDropZone.classList.remove('active');
    }

    if (state.draggedTab.fromPane === 'left') {
      if (rightPane.style.display !== 'none') {
        const midRightPane = rightRect.left + rightRect.width / 2;

        if (e.clientX >= rightRect.left && e.clientX <= midRightPane &&
            e.clientY >= rightRect.top && e.clientY <= rightRect.bottom) {
          rightDropZone.style.display = 'flex';
          rightDropZone.style.pointerEvents = 'auto';
          rightDropZone.classList.add('active');
        } else {
          rightDropZone.style.display = 'none';
          rightDropZone.style.pointerEvents = 'none';
          rightDropZone.classList.remove('active');
        }
      } else {
        if (e.clientX >= midLeftPane && e.clientX <= leftRect.right &&
            e.clientY >= leftRect.top && e.clientY <= leftRect.bottom) {
          leftDropZone.style.display = 'flex';
          leftDropZone.style.pointerEvents = 'auto';
          leftDropZone.classList.add('active');
        } else {
          leftDropZone.style.display = 'none';
          leftDropZone.style.pointerEvents = 'none';
          leftDropZone.classList.remove('active');
        }
      }
    }

    e.preventDefault();
  });

  document.addEventListener('drop', (e) => {
    if (!state.draggedTab) return;
    e.preventDefault();
    hideDropZones();
    setPdfPointerEvents(true);
  });

  document.addEventListener('dragend', () => {
    hideDropZones();
    setPdfPointerEvents(true);
  });
}
