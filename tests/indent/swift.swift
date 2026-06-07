import Foundation

protocol Shape {
  var area: Double { get }
  func describe() -> String
}

class Rectangle: Shape {
  var width: Double
  var height: Double

  var area: Double {
    return width * height
  }

  var perimeter: Double {
    get {
      return 2 * (width + height)
    }
    set {
      width = newValue / 4
    }
  }

  var observed: Int = 0 {
    willSet {
      print(newValue)
    }
    didSet {
      print(oldValue)
    }
  }

  init(width: Double, height: Double) {
    self.width = width
    self.height = height
  }

  func describe() -> String {
    return "rectangle"
  }

  func compute<T: Numeric>(values: [T]) -> [T] {
    let doubled = values.map { value in
      return value + value
    }
    return doubled
  }
}

enum Direction {
  case north
  case south
  case east
  case west
}

func process(_ direction: Direction, count: Int) {
  guard count > 0 else {
    return
  }

  let numbers = [
    1,
    2,
    3,
  ]

  let mapping = [
    "a": 1,
    "b": 2,
  ]

  for number in numbers {
    if number > 1 {
      print(number)
    } else if number == 1 {
      print("one")
    } else {
      print("zero")
    }
  }

  var index = 0
  while index < count {
    index += 1
  }

  repeat {
    index -= 1
  } while index > 0

  switch direction {
    case .north:
      let label = "up"
      print(label)
    case .south where count > 5:
      print("down")
    default:
      print("side")
  }

  let result = compute(width: 10,
    height: 20)
  print(result)
}
