resource "aws_instance" "web" {
#                              ^ 2 eof
  tags = {}
#         ^ 2
  ami = "abc"
  tags = {
    Name = "web"
  }
}
