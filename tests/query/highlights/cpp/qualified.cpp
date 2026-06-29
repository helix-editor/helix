void g(int x) {
  auto a = Color::Red;
//                ^ @type.enum.variant
  auto b = Limits::MAX;
//                 ^ @type.enum.variant
  std::cout << x;
//     ^ @variable
}
enum class Status { ON };
//                  ^ @type.enum.variant
