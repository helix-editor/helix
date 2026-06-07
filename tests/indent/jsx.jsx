function App() {
  return (
    <div className="container">
      <Header title="hello" />
      <ul>
        {items.map((item) => (
          <li key={item.id}>{item.name}</li>
        ))}
      </ul>
      <button onClick={() => handleClick()}>Click</button>
    </div>
  );
}

const value = store
  .getState()
  .filter((x) => x.active);

function List({ items, show }) {
  return (
    <>
      {show && <Banner />}
      {items.map((item) => (
        <Item key={item.id} value={item.value} />
      ))}
      <Footer
        copyright="2024"
        links={links}
      />
    </>
  );
}

function Card({ id, title }) {
  return (
    <article
      className="card"
      data-id={id}
    >
      <h2>{title}</h2>
    </article>
  );
}
