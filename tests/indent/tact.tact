contract Counter {
    value: Int;

    fun add(x: Int) {
        if (x > 0) {
            self.value = self.value + x;
        } else {
            self.value = 0;
        }
    }
}
