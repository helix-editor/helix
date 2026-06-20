namespace App
{
	public enum Color
	{
		Red,
		Green,
	}

	public class Counter
	{
		private int count;

		public int Bump()
		{
			if (count > 0)
			{
				count += 1;
			}
			else
			{
				count = 0;
			}
			return count;
		}

		public string Classify(int x)
		{
			switch (x)
			{
				case 1:
					return "one";
				default:
					return "many";
			}
		}
	}
}
