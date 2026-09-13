export async function live(): Promise<void> {
    try {
        await read();
    } catch {
    }

    await read().catch(() => {});
    await read().catch((error) => recover(error));
}

// @ts-ignore
const legacy = read();

// eslint-disable-next-line no-console
console.log(legacy);

function widen(value: unknown): unknown {
    return value as any;
}

function asserted(): void {
    expect(1).toBe(1);
}
