resource "aws_instance" "web" {
#                              ^ 2 eof
  ami = "abc"
  tags = {
    Name = "web"
  }
}
