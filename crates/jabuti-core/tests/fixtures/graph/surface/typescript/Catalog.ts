function internal(value: string): void {
    const shown = value.trim();
    console.log(shown);
}

export class Catalog {
    readonly label = "catalog";
    private secret = "hidden";

    public listed(): string {
        const value = this.label;
        return value;
    }

    private hidden(): string {
        const value = this.secret;
        return value;
    }

    @Controller()
    endpoint(): string {
        const result = this.listed();
        return result;
    }
}

export const factory = (label: string): Catalog => {
    const catalog = new Catalog();
    catalog.label.concat(label);
    return catalog;
};
