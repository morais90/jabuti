fun live() {
    val required = read()!!
    val optional = runCatching { read() }.getOrNull()
    try {
        read()
    } catch (error: Exception) {
    }
}

class LiveTest {
    @Test
    fun checks() {
        val hidden = read()!!
    }
}

@Suppress("UNCHECKED_CAST")
fun unused() {}
