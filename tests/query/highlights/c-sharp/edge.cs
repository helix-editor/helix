class C {
    void m() {
        var x = $"{obj.Compute()}";
//                     ^ @function
    }
}
