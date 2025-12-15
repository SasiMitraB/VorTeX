export const hideDropZones = () => {
  document.querySelectorAll('.drop-zone').forEach(zone => {
    zone.classList.remove('active');
    zone.style.pointerEvents = 'none';
    zone.style.display = 'none';
  });
};

export const setPdfPointerEvents = (enabled) => {
  document.querySelectorAll('.pdf-frame').forEach(f => {
    f.style.pointerEvents = enabled ? 'auto' : 'none';
  });
};
