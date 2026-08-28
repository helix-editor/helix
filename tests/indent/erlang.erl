-module(mymod).
-export([process/1, classify/1, loop/0]).

process(Items) ->
    Total = lists:foldl(
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
        error:Reason ->
            {error, Reason}
    after
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
