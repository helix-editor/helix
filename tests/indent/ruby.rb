module Greeter
  class Hello
    def initialize(name)
      @name = name
    end

    def greet
      if @name
        puts "Hello, #{@name}"
      elsif @other
        puts "Hi"
      else
        puts "Hello"
      end
    end

    def classify(x)
      case x
      when 1
        "one"
      else
        "many"
      end
    end

    def safe
      begin
        risky
      rescue => e
        handle(e)
      ensure
        cleanup
      end
    end

    def each_item
      [1, 2, 3].each do |item|
        process(item)
      end
    end
  end
end
