package shop

@RestController
class CatalogController(private val shelf: Shelf) {
    fun list() = shelf.books()
}

class Shelf {
    fun books() = listOf<String>()
}

class Basket

class Crate

object Registry

internal class Ledger

fun main() {
    register(Registry)
}
