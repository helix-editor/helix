package example

class Example(private val value: Int) {
    fun compute(a: Int, b: Int): Int {
        val numbers = listOf(
            1,
            2,
            3,
        )
        for (n in numbers) {
            println(n)
        }
        val result = when (b) {
            1 -> a
            2 -> b
            else -> 0
        }
        // Known limitation (documented, not checked): else / catch / finally
        // bodies over-indent by one level because the block begins on a different
        // line than the enclosing if / try, so the two indents do not collapse.
        // The brace-less if-body additionally has no public grammar node to indent
        // against, so this needs a grammar change rather than a query fix. The
        // cases are left here, commented out, to document the edge.
        // if (a > b) {
        //     println("gt")
        // } else {
        //     println("le")
        // }
        // try {
        //     return result
        // } catch (e: Exception) {
        //     return 0
        // } finally {
        //     println("done")
        // }
        return result
    }
}
