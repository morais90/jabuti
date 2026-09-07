export async function live(): Promise<void> {
    try {
        await read();
    } catch {
    }

    await read().catch(() => {});
    await read().catch((error) => recover(error));
}
