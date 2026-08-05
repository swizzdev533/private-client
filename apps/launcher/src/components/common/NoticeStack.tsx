import { AnimatePresence, motion } from "framer-motion";
import { AlertTriangle, CheckCircle2, Info, X, XCircle } from "lucide-react";
import { useUiStore, type Notice } from "../../stores/useUiStore";

const icons: Readonly<Record<Notice["tone"], typeof Info>> = {
  neutral: Info,
  success: CheckCircle2,
  warning: AlertTriangle,
  error: XCircle,
};

export function NoticeStack() {
  const notices = useUiStore((state) => state.notices);
  const dismiss = useUiStore((state) => state.dismissNotice);

  return (
    <aside className="notice-stack" aria-label="Powiadomienia" aria-live="polite">
      <AnimatePresence initial={false}>
        {notices.map((notice) => {
          const Icon = icons[notice.tone];
          return (
            <motion.article
              key={notice.id}
              className={`notice notice--${notice.tone}`}
              initial={{ opacity: 0, x: 24, scale: 0.97 }}
              animate={{ opacity: 1, x: 0, scale: 1 }}
              exit={{ opacity: 0, x: 16, scale: 0.98 }}
              layout
            >
              <Icon size={18} aria-hidden="true" />
              <div className="notice__copy">
                <strong>{notice.title}</strong>
                <p>{notice.message}</p>
              </div>
              <button
                type="button"
                className="notice__close"
                onClick={() => {
                  dismiss(notice.id);
                }}
                aria-label="Zamknij powiadomienie"
              >
                <X size={14} />
              </button>
            </motion.article>
          );
        })}
      </AnimatePresence>
    </aside>
  );
}
