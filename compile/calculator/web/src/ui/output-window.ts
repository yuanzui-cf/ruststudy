export class OutputWindow {
  constructor(
    private readonly container: HTMLElement,
    private readonly limit = 10_000,
  ) {}

  clear(): void {
    this.container.replaceChildren();
  }

  append(entries: readonly string[], className = "output-entry"): void {
    const fragment = document.createDocumentFragment();
    for (const entry of entries) {
      const row = document.createElement("div");
      row.className = className;
      row.textContent = entry;
      fragment.append(row);
    }
    this.container.append(fragment);

    while (this.container.childElementCount > this.limit) {
      this.container.firstElementChild?.remove();
    }
    this.container.scrollTop = this.container.scrollHeight;
  }
}
