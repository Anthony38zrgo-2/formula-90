// Selection state for the viewport (which object is active).
export class Selection {
    constructor() {
        this.ids = new Set();
    }
    has(id) {
        return this.ids.has(id);
    }
    size() {
        return this.ids.size;
    }
    select(id) {
        this.ids.add(id);
    }
    deselect(id) {
        this.ids.delete(id);
    }
    clear() {
        this.ids.clear();
    }
    all() {
        return Array.from(this.ids);
    }
}
export const selection = new Selection();
