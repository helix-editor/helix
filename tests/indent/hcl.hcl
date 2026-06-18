resource "aws_instance" "web" {
  ami = "abc"
  tags = {
    Name = "web"
  }
}
