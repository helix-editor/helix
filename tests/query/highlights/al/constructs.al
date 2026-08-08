table 50100 "My Table"
//    ^ @constant
{
    Caption = 'My Table';
//  ^ @variable.other.member

    fields
    {
        field(1; Name; Text[50])
        {
            NotBlank = true;
//          ^ @variable.other.member
        }
    }

    procedure Compute(Value: Decimal): Decimal
//            ^ @function
//                    ^ @variable.parameter
    var
        Total: Decimal;
    begin
        // a line comment
//      ^ @comment.line
        Total := Value * 2;
        exit(Total);
    end;
}
