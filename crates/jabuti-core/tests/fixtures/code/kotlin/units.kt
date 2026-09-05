package demo

class Config(val depth: Int) {
    fun doubled(): Int {
        val scale = { value: Int -> value * 2 }
        return scale(depth)
    }
}

object Registry {
    fun register(name: String) {}
}

interface Visitor {
    fun visit(depth: Int): Boolean = depth > 0
}

fun outer(): Int {
    fun inner(): Int = 1
    return inner()
}
