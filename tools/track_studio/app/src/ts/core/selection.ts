// Selection state for the viewport (which object is active).

export class Selection {
  private readonly ids = new Set<string>();

  has(id: string): boolean {
    return this.ids.has(id);
  }

  size(): number {
    return this.ids.size;
  }

  select(id: string): void {
    this.ids.add(id);
  }

  deselect(id: string): void {
    this.ids.delete(id);
  }

  clear(): void {
    this.ids.clear();
  }

  all(): string[] {
    return Array.from(this.ids);
  }
}

export const selection = new Selection();
