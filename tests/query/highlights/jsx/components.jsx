function App(props) {
  const x = obj.field;
//              ^ @variable.other.member
  return <Button id={x} />;
//        ^ @constructor
//               ^ @attribute
}
