interface Props {
  title: string;
  count: number;
}

function Widget({ title, count }: Props): JSX.Element {
  if (a) {}
//        ^ 2
  return (
    <div className="widget">
      <span>{title}</span>
      {count > 0 && (
        <ul>
          {Array.from({ length: count }).map((_, i) => (
            <li key={i}>{i}</li>
          ))}
        </ul>
      )}
    </div>
  );
}

const data = service
  .fetch()
  .then((res) => res.json());
