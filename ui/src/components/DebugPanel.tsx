import React, { useEffect, useRef } from 'react';
import type { ExecutionEvent } from '../types';

interface DebugPanelProps {
    events: ExecutionEvent[];
}

export const DebugPanel: React.FC<DebugPanelProps> = ({ events }) => {
    const endRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        endRef.current?.scrollIntoView({ behavior: 'smooth' });
    }, [events]);

    return (
        <div className="debug-panel" style={{
            width: '300px',
            borderLeft: '1px solid #ccc',
            padding: '10px',
            overflowY: 'auto',
            backgroundColor: '#f5f5f5',
            fontSize: '12px',
            fontFamily: 'monospace'
        }}>
            <h3>Execution Log</h3>
            {events.length === 0 && <div style={{ color: '#888' }}>Waiting for events...</div>}
            {events.map((event, i) => (
                <div key={i} style={{ marginBottom: '10px', padding: '5px', background: '#fff', border: '1px solid #ddd', borderRadius: '4px' }}>
                    <div style={{ fontWeight: 'bold', color: getColor(event.type) }}>
                        [{new Date().toLocaleTimeString()}] {event.type}
                    </div>
                    {event.node_id && <div>Node: {event.node_id}</div>}
                    {event.from && <div>From: {event.from} -&gt; To: {event.to}</div>}
                    {event.value && (
                        <pre style={{ margin: '5px 0', overflowX: 'auto' }}>
                            {JSON.stringify(event.value, null, 2)}
                        </pre>
                    )}
                    {event.error && <div style={{ color: 'red' }}>Error: {event.error}</div>}
                </div>
            ))}
            <div ref={endRef} />
        </div>
    );
};

function getColor(type: string) {
    switch (type) {
        case 'NodeStart': return 'blue';
        case 'NodeFinish': return 'green';
        case 'EdgeData': return 'purple';
        case 'NodeError': return 'red';
        default: return 'black';
    }
}
