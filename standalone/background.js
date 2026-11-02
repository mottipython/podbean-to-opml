const api = globalThis.browser ?? globalThis.chrome;
api.action.onClicked.addListener(async () => {
  const url = api.runtime.getURL('app.html');
  const [open] = await api.tabs.query({ url });
  if (open) {
    await api.tabs.update(open.id, { active: true });
    await api.windows.update(open.windowId, { focused: true });
  } else {
    await api.tabs.create({ url });
  }
});
