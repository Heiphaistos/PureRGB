// Historique annuler/refaire par instantanés JSON (pur, sans Vue).

export class History<T> {
  private stack: string[];
  private index = 0;
  private lastKey: string | null = null;
  private lastAt = 0;
  private readonly limit: number;
  private readonly coalesceMs: number;

  constructor(initial: T, limit = 100, coalesceMs = 700) {
    this.stack = [JSON.stringify(initial)];
    this.limit = limit;
    this.coalesceMs = coalesceMs;
  }

  /** Enregistre un état. Même `key` en rafale (saisie) = remplace l'entrée précédente. */
  push(state: T, key: string | null = null, now = Date.now()): boolean {
    const json = JSON.stringify(state);
    if (json === this.stack[this.index]) return false;
    const coalesce = key !== null && key === this.lastKey && now - this.lastAt < this.coalesceMs && this.index > 0;
    this.stack.length = this.index + 1;
    if (coalesce) {
      this.stack[this.index] = json;
    } else {
      this.stack.push(json);
      this.index++;
      if (this.stack.length > this.limit) {
        this.stack.shift();
        this.index--;
      }
    }
    this.lastKey = key;
    this.lastAt = now;
    return true;
  }

  get canUndo(): boolean {
    return this.index > 0;
  }

  get canRedo(): boolean {
    return this.index < this.stack.length - 1;
  }

  undo(): T | null {
    if (!this.canUndo) return null;
    this.index--;
    this.lastKey = null;
    return JSON.parse(this.stack[this.index]) as T;
  }

  redo(): T | null {
    if (!this.canRedo) return null;
    this.index++;
    this.lastKey = null;
    return JSON.parse(this.stack[this.index]) as T;
  }

  reset(state: T): void {
    this.stack = [JSON.stringify(state)];
    this.index = 0;
    this.lastKey = null;
  }
}
