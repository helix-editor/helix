class C {
    int m() {
        invokeit();
//      ^ @function.method
        obj.doThing();
//          ^ @function.method
        return 0;
    }
}
