namespace MyCompany.Sales;

using Microsoft.Foundation.Address;

table 50100 "My Table"
{
    Caption = 'My Table';

    fields
    {
        field(1; Name; Text[50])
        {
            Caption = 'Name';
            NotBlank = true;
        }
        field(2; Amount; Decimal)
        {
            Caption = 'Amount';
        }
    }

    keys
    {
        key(PK; Name)
        {
            Clustered = true;
        }
    }

    trigger OnInsert()
    var
        Customer: Record Customer;
        Total: Decimal;
    begin
        Total := 0;
        if Amount > 0 then
            Total := Amount
        else
            Total := 1;

        if Amount > 100 then begin
            Total := Amount * 2;
            Customer.Init();
        end;

        while Total > 0 do
            Total -= 1;

        for Total := 1 to 10 do begin
            Customer.Init();
            Customer.Insert();
        end;

        repeat
            Total += 1;
        until Total > 5;

        case Amount of
            0:
                Total := 0;
            1:
                begin
                    Total := 1;
                    Customer.Init();
                end;
            else
                Total := 99;
        end;
    end;

    procedure Compute(Value: Decimal): Decimal
    var
        Result: Decimal;
    begin
        Result := Value * 2;
        exit(Result);
    end;
}

codeunit 50100 "My Codeunit"
{
    procedure DoWork()
    begin
        Message('working');
    end;
}
