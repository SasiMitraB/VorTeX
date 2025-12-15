export const state = {
  editors: { left: null, right: null },
  monaco: null,
  paneTabs: { left: [], right: [] },
  paneState: { left: { activeId: null }, right: { activeId: null } },
  activePane: 'left',
  nextUntitled: 1,
  draggedTab: null
};

export const newTabId = () => Math.random().toString(36).slice(2, 9);

export const setActivePane = (pane) => {
  state.activePane = pane;
};
