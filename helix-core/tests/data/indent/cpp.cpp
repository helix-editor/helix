std::vector<std::string>
fn_with_many_parameters(int parm1, long parm2, float parm3, double parm4,
                        char* parm5, bool parm6);

std::vector<std::string>
fn_with_many_parameters(int parm1, long parm2, float parm3, double parm4,
                        char* parm5, bool parm6) {
  auto lambda = []() {
    return 0;
  };
  auto lambda_with_a_really_long_name_that_uses_a_whole_line
    = [](int some_more_aligned_parameters,
         std::string parm2) {
      do_smth();
    };
  if (brace_on_same_line) {
    do_smth();
  } else if (brace_on_next_line)
  {
    do_smth();
  } else if (another_condition) {
    do_smth();
  }
  else {
    do_smth();
  }
  if (inline_if_statement)
    do_smth();
  if (another_inline_if_statement)
    return [](int parm1, char* parm2) {
      this_is_a_really_pointless_lambda();
    };

  switch (var) {
  case true:
    return -1;
  case false:
    return 42;
  }
}

class MyClass : public MyBaseClass {
public:
  MyClass();
  void public_fn();
private:
  super_secret_private_fn();
}
