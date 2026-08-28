module M

function process(items)
    total = 0
    for v in items
        if v > 0
            total += v
        elseif v < 0
            total -= v
        else
            total += 1
        end
    end
    config = Dict(
        "a" => 1,
        "b" => 2,
    )
    return total
end

function safe(x)
    try
        risky(x)
    catch e
        handle(e)
    finally
        cleanup()
    end
end

end
