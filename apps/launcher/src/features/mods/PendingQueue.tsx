import { Clock3, ListRestart, Trash2 } from "lucide-react";
import { Button } from "../../components/common/Button";
import { Badge } from "../../components/common/Badge";
import { formatDate } from "../../lib/format";
import type { PendingOperation } from "../../types/contracts";

interface PendingQueueProps {
  operations: PendingOperation[];
  onCancel: (id: string) => void;
  onApply: () => void;
}

export function PendingQueue({ operations, onCancel, onApply }: PendingQueueProps) {
  if (operations.length === 0) {
    return null;
  }

  return (
    <section className="pending-queue">
      <header>
        <div>
          <span className="eyebrow">BEZPIECZNA KOLEJKA</span>
          <h3>Zmiany oczekujące na zamknięcie gry</h3>
        </div>
        <Button
          size="sm"
          variant="secondary"
          icon={<ListRestart size={14} />}
          onClick={onApply}
        >
          ZASTOSUJ
        </Button>
      </header>
      <div className="pending-queue__items">
        {operations.map((operation) => (
          <article key={operation.id}>
            <span className="pending-queue__type">{operation.type}</span>
            <div>
              <strong>{operation.targetName}</strong>
              <span>
                <Clock3 size={11} />
                {formatDate(operation.createdAt)}
              </span>
            </div>
            <Badge tone={operation.status === "FAILED" ? "danger" : "warning"}>
              {operation.status}
            </Badge>
            <Button
              size="icon"
              variant="ghost"
              icon={<Trash2 size={14} />}
              aria-label={`Usuń ${operation.targetName} z kolejki`}
              onClick={() => {
                onCancel(operation.id);
              }}
            />
          </article>
        ))}
      </div>
    </section>
  );
}
