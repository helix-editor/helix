module Greeter
  class Hello
    def greet(name)
      if name
        puts "Hello"
      elsif other
        puts "Hi"
      else
        puts "Hey"
      end
    end

    def classify(x)
#                  ^ 6 eof
      case x
      when 1
        "one"
      else
        "many"
      end
    end
  end
end
