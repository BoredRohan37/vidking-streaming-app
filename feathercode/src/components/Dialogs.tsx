import { useState } from "react";

export function InputDialog({
  title,
  initial,
  placeholder,
  onOk,
  onCancel,
}: {
  title: string;
  initial?: string;
  placeholder?: string;
  onOk: (value: string) => void;
  onCancel: () => void;
}) {
  const [v, setV] = useState(initial ?? "");
  return (
    <div className="modal-backdrop" onMouseDown={onCancel}>
      <div className="modal small" onMouseDown={(e) => e.stopPropagation()}>
        <div className="modal-title">{title}</div>
        <input
          autoFocus
          value={v}
          placeholder={placeholder}
          onChange={(e) => setV(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") onOk(v);
            if (e.key === "Escape") onCancel();
          }}
        />
        <div className="modal-actions">
          <button onClick={onCancel}>Cancel</button>
          <button className="primary" onClick={() => onOk(v)}>
            OK
          </button>
        </div>
      </div>
    </div>
  );
}

export function ConfirmDialog({
  message,
  onOk,
  onCancel,
}: {
  message: string;
  onOk: () => void;
  onCancel: () => void;
}) {
  return (
    <div className="modal-backdrop" onMouseDown={onCancel}>
      <div className="modal small" onMouseDown={(e) => e.stopPropagation()}>
        <div className="modal-title">Confirm</div>
        <div style={{ color: "var(--muted)", marginTop: 6 }}>{message}</div>
        <div className="modal-actions">
          <button onClick={onCancel}>Cancel</button>
          <button className="danger" onClick={onOk}>
            Delete
          </button>
        </div>
      </div>
    </div>
  );
}
