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
        if (a > b) {
            println("gt")
        } else {
            println("le")
        }
        // Known limitation (documented, not checked): catch / finally
        // bodies over-indent by one level because the block begins on a different
        // line than the enclosing try, so the two indents do not collapse.
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

val multi = """
unindented
    indented
"""

fun conditionals(a: Int, b: Int) {
    if (a > b)
        println("gt")
    else if (a == b)
        println("eq")
    else
        println("lt")

    if (
        a > b
    ) {
        println("gt")
    } else if (
        a == b
    ) {
        println("eq")
    } else {
        if (a > 0) {
            println("positive")
        } else {
            println("nonpositive")
        }
    }

    if (a > b) {
        if (b > 0) {
            println("positive")
        } else {
            println("nonpositive")
        }
    } else {
        println("le")
    }

    val braced = if (a > b) {
        a
    } else {
        b
    }
    val unbraced = if (a > b)
        a
    else
        b
    val continued =
        if (a > b) {
            a
        } else {
            b
        }
    var assigned = a
    assigned = if (a > b) {
        a
    } else {
        b
    }
}
