package org.example.storage

import org.example.catalog.Shelf

class Repository(private val shelf: Shelf) {
    fun best(): String = shelf.longest().title
}
