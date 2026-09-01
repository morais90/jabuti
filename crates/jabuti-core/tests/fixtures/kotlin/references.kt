package org.example.catalog

import org.example.storage.BookRepository
import org.example.service.BookLifecycle as Lifecycle
import java.time.LocalDateTime

typealias BookList = List<Book>

class CatalogService(private val repository: BookRepository) {
    fun latest(limit: Int): BookList {
        repository.findAll(limit)
        val search = BookSearch(limit)
        val stamp = LocalDateTime.now()

        return Shelf.arrange(search, stamp)
    }
}

object Registry {
    fun lifecycle(): Lifecycle = Lifecycle()
}

fun describe(author: Author): String = author.name
