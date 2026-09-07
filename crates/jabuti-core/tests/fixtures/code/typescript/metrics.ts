export function straight(value: number): number {
    return value;
}

export function branches(value: number): string {
    if (value > 0) {                 // cyclomatic +1, cognitive +1
        return "positive";
    } else if (value < 0) {          // cyclomatic +1, cognitive +1
        return "negative";
    } else {                         // cognitive +1
        return "zero";
    }                                // cyclomatic = 3, cognitive = 3
}

export function nested(first: boolean, second: boolean): boolean {
    if (first) {                     // cyclomatic +1, cognitive +1
        if (second) {                // cyclomatic +1, cognitive +2 at nesting 1
            return true;
        }
    }                                // cyclomatic = 3, cognitive = 3

    return false;
}

export function logical(a: boolean, b: boolean, c: boolean): boolean {
    if (a && b || c) {               // cyclomatic +3, cognitive +3
        return true;
    }                                // cyclomatic = 4, cognitive = 3

    return false;
}

export function loops(values: number[]): number {
    let total = 0;
    for (let index = 0; index < values.length; index += 1) { // +1 for
        while (total < values[index]) {                       // +1 cyclomatic, +2 cognitive at nesting 1
            total += 1;
        }
    }
    for (const value of values) {                             // +1 for-of
        total += value;
    }
    do {                                                      // +1 do
        total -= 1;
    } while (total > 100);
    try {
        return total;
    } catch {                                                 // +1 catch
        return 0;
    }                                                         // cyclomatic = 6, cognitive = 6
}

export function choose(active: boolean, first: string, second: string): string {
    return active ? first : second;   // cyclomatic +1, cognitive +1
}                                    // cyclomatic = 2, cognitive = 1

export function dispatch(value: number): string {
    switch (value) {                  // cognitive +1
        case 1:                       // cyclomatic +1
            return "one";
        case 2:                       // cyclomatic +1
            return "two";
        default:
            return "other";
    }                                 // cyclomatic = 3, cognitive = 1
}

export function coalesce(value: string | undefined): string {
    return value ?? "fallback";      // cyclomatic +1, cognitive +1
}                                    // cyclomatic = 2, cognitive = 1

export class Receiver {
    compare(this: Receiver, value: number): boolean {
        return this === value;
    }
}
