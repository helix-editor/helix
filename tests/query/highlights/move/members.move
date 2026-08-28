module a::m {
    fun f(o: S) {
        let x = o.val;
//                ^ @variable.other.member
        let y = vector::length(&o);
//                      ^ @function
    }
}
