-module(mymod).
-export([process/1, classify/1, loop/0]).

process(Items) ->
%                ^ 4 eof
    Total = lists:foldl(
%                       ^ 8 eof
        fun(X, Acc) ->
            X + Acc
        end,
        0,
        Items
    ),
    case Total of
        0 ->
            zero;
        _ ->
            many
    end.

classify(X) ->
    if
        X > 0 ->
            positive;
        true ->
            other
    end.

loop() ->
    receive
        {msg, Data} ->
            handle(Data);
        stop ->
            ok
    end.

safe(F) ->
    try F() of
        Result ->
            Result
    catch
%        ^ 8 eof
        error:Reason ->
            {error, Reason}
    after
%        ^ 8 eof
        cleanup()
    end.

build() ->
    #{
        name => "test",
        items => [
            1,
            2
        ]
    }.
