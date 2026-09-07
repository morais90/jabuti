import { Shelf as CatalogShelf } from "../catalog/Shelf.js";

export class Repository {
    constructor(private readonly shelf: CatalogShelf) {}
}
