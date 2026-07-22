defmodule Greeter do
  def greet(name) do
    if name do
      IO.puts("Hello")
    else
      IO.puts("Hi")
    end
  end

  def classify(x) do
    case x do
      1 -> "one"
      2 -> "two"
      _ -> "many"
    end
  end

  def safe(x) do
    try do
      risky(x)
    rescue
      e -> handle(e)
    after
      cleanup()
    end
  end

  def transform(list) do
    list
    |> Enum.map(fn x -> x * 2 end)
    |> Enum.filter(fn x -> x > 0 end)
  end

  def config do
    %{
      name: "test",
      values: [
        1,
        2,
      ]
    }
  end

  def rank(x) do
    cond do
      x > 10 ->
        :high
      true ->
        :low
    end
  end
end
