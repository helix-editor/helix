object O {
  def m() = {
    invokeit()
//  ^ @function
    obj.doThing()
//      ^ @function.method
    val x = obj.field
//              ^ @variable.other.member
  }
}
