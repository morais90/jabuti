export interface Store {
    fetch(id: string): Promise<Item | undefined>;
}

export type Item = {
    id: string;
    label: string;
};

export class Catalog {
    constructor(private readonly store: Store) {}

    async display(id: string): Promise<string> {
        const label = (item: Item | undefined): string => {
            const shown = item?.label ?? "missing";
            return shown;
        };

        function normalise(value: string): string {
            const trimmed = value.trim();
            return trimmed;
        }

        return normalise(label(await this.store.fetch(id)));
    }
}

export function outer(value: number): number {
    function inner(candidate: number): number {
        const doubled = candidate * 2;
        return doubled;
    }

    return inner(value);
}
