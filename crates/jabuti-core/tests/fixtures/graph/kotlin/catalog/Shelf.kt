package org.example.catalog

class Shelf(private val books: List<Book>) {
    fun longest(): Book = books.maxBy { it.pages }
}
