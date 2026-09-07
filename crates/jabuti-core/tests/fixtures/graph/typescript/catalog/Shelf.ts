import { Book } from "./Book";

export class Shelf {
    label(book: Book): string {
        const shown = book.title.trim();
        return shown;
    }
}
