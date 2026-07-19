export class OutputBatcher {
  readonly #entries: string[] = [];

  constructor(
    private readonly emit: (entries: string[]) => void,
    private readonly batchSize = 100,
  ) {}

  push(entry: string): void {
    this.#entries.push(entry);
    if (this.#entries.length >= this.batchSize) {
      this.flush();
    }
  }

  flush(): void {
    if (this.#entries.length === 0) {
      return;
    }
    this.emit(this.#entries.splice(0));
  }
}
