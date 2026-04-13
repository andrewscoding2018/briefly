import { bootstrapCards } from "@/lib/contracts";

export default function HomePage() {
  return (
    <main className="page">
      <section className="hero">
        <p className="eyebrow">Briefly Bootstrap</p>
        <h1>Workspace skeleton for the desktop shell, Rust services, and shared contracts.</h1>
        <p className="lede">
          This is an intentional placeholder surface while ingestion, scoring, and
          persistence move from docs into executable code.
        </p>
      </section>

      <section className="grid" aria-label="Bootstrap boundaries">
        {bootstrapCards.map((card) => (
          <article className="card" key={card.title}>
            <h2>{card.title}</h2>
            <p>{card.description}</p>
          </article>
        ))}
      </section>
    </main>
  );
}
