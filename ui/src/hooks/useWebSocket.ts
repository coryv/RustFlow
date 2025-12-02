import { useState, useEffect, useRef, useCallback } from 'react';
import type { ExecutionEvent } from '../types';

interface UseWebSocketReturn {
    isConnected: boolean;
    lastEvent: ExecutionEvent | null;
    connect: (jobId: string) => void;
    disconnect: () => void;
}

export const useWebSocket = (): UseWebSocketReturn => {
    const [isConnected, setIsConnected] = useState(false);
    const [lastEvent, setLastEvent] = useState<ExecutionEvent | null>(null);
    const socketRef = useRef<WebSocket | null>(null);

    const connect = useCallback((jobId: string) => {
        if (socketRef.current) {
            socketRef.current.close();
        }

        const ws = new WebSocket(`ws://localhost:3000/api/ws/${jobId}`);

        ws.onopen = () => {
            console.log('WebSocket Connected');
            setIsConnected(true);
        };

        ws.onmessage = (event) => {
            try {
                const data = JSON.parse(event.data);
                // Backend sends enum variants like {"NodeStart": {...}} or {"EdgeData": {...}}
                // We need to normalize this to our flat interface
                let normalizedEvent: ExecutionEvent | null = null;

                if (data.NodeStart) {
                    normalizedEvent = { type: 'NodeStart', node_id: data.NodeStart.node_id };
                } else if (data.NodeFinish) {
                    normalizedEvent = { type: 'NodeFinish', node_id: data.NodeFinish.node_id };
                } else if (data.EdgeData) {
                    normalizedEvent = {
                        type: 'EdgeData',
                        from: data.EdgeData.from,
                        to: data.EdgeData.to,
                        value: data.EdgeData.value
                    };
                } else if (data.NodeError) {
                    normalizedEvent = {
                        type: 'NodeError',
                        node_id: data.NodeError.node_id,
                        error: data.NodeError.error
                    };
                }

                if (normalizedEvent) {
                    setLastEvent(normalizedEvent);
                }
            } catch (e) {
                console.error('Failed to parse WebSocket message:', e);
            }
        };

        ws.onclose = () => {
            console.log('WebSocket Disconnected');
            setIsConnected(false);
        };

        ws.onerror = (error) => {
            console.error('WebSocket Error:', error);
        };

        socketRef.current = ws;
    }, []);

    const disconnect = useCallback(() => {
        if (socketRef.current) {
            socketRef.current.close();
            socketRef.current = null;
            setIsConnected(false);
        }
    }, []);

    useEffect(() => {
        return () => {
            if (socketRef.current) {
                socketRef.current.close();
            }
        };
    }, []);

    return { isConnected, lastEvent, connect, disconnect };
};
