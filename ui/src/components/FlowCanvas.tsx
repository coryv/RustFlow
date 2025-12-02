import React, { useCallback, useRef, useState, useEffect } from 'react';
import {
    ReactFlow,
    MiniMap,
    Controls,
    Background,
    addEdge,
    type Connection,
    type Edge,
    type Node,
    type NodeTypes,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { CustomNode } from './CustomNode';
import { type NodeType } from '../types';

const nodeTypes: NodeTypes = {
    workflowNode: CustomNode,
};

interface FlowCanvasProps {
    onNodeSelect: (node: Node | null) => void;
    nodes: Node[];
    edges: Edge[];
    onNodesChange: any;
    onEdgesChange: any;
    setNodes: any;
    setEdges: any;
    edgeActivity?: Record<string, number>;
    nodeStatus?: Record<string, string>;
}

export const FlowCanvas: React.FC<FlowCanvasProps> = ({
    onNodeSelect,
    nodes,
    edges,
    onNodesChange,
    onEdgesChange,
    setNodes,
    setEdges,
    edgeActivity = {},
    nodeStatus = {},
}) => {
    const reactFlowWrapper = useRef<HTMLDivElement>(null);
    const [reactFlowInstance, setReactFlowInstance] = useState<any>(null);

    // Effect to handle node status updates
    useEffect(() => {
        setNodes((nds: Node[]) =>
            nds.map((node) => {
                const status = nodeStatus[node.id];
                let style = { ...node.style };

                if (status === 'running') {
                    style = { ...style, border: '2px solid #007bff', boxShadow: '0 0 10px #007bff' };
                } else if (status === 'completed') {
                    style = { ...style, border: '2px solid #28a745', boxShadow: 'none' };
                } else if (status === 'failed') {
                    style = { ...style, border: '2px solid #dc3545', boxShadow: 'none' };
                } else {
                    // Reset or default
                    style = { ...style, border: '1px solid #777', boxShadow: 'none' };
                }

                return { ...node, style };
            })
        );
    }, [nodeStatus, setNodes]);

    // Effect to handle edge highlighting
    useEffect(() => {
        if (Object.keys(edgeActivity).length === 0) return;

        setEdges((eds: Edge[]) =>
            eds.map((edge) => {
                const key = `${edge.source}-${edge.target}`;
                const lastActive = edgeActivity[key];

                // If active in last 2 seconds, highlight
                if (lastActive && Date.now() - lastActive < 2000) {
                    return {
                        ...edge,
                        animated: true,
                        style: { ...edge.style, stroke: '#ff0072', strokeWidth: 3 },
                    };
                }

                // Reset style if not active
                return {
                    ...edge,
                    animated: false,
                    style: { ...edge.style, stroke: '#b1b1b7', strokeWidth: 1 },
                };
            })
        );
    }, [edgeActivity, setEdges]);

    // Effect to clear edge highlighting after 2 seconds
    useEffect(() => {
        if (Object.keys(edgeActivity).length === 0) return;

        const interval = setInterval(() => {
            setEdges((eds: Edge[]) =>
                eds.map((edge) => {
                    const key = `${edge.source}-${edge.target}`;
                    const lastActive = edgeActivity[key];

                    // If active in last 2 seconds, keep highlight
                    if (lastActive && Date.now() - lastActive < 2000) {
                        return edge; // No change needed if already highlighted correctly
                    }

                    // If it WAS highlighted but shouldn't be anymore, reset it
                    if (edge.animated) {
                        return {
                            ...edge,
                            animated: false,
                            style: { ...edge.style, stroke: '#b1b1b7', strokeWidth: 1 },
                        };
                    }

                    return edge;
                })
            );
        }, 500); // Check every 500ms

        return () => clearInterval(interval);
    }, [edgeActivity, setEdges]);

    const onConnect = useCallback(
        (params: Connection) => setEdges((eds: Edge[]) => addEdge(params, eds)),
        [setEdges],
    );

    const onDragOver = useCallback((event: React.DragEvent) => {
        event.preventDefault();
        event.dataTransfer.dropEffect = 'move';
    }, []);

    const onDrop = useCallback(
        (event: React.DragEvent) => {
            event.preventDefault();

            const typeData = event.dataTransfer.getData('application/reactflow');
            if (!typeData) return;

            const nodeType: NodeType = JSON.parse(typeData);

            // check if the dropped element is valid
            if (typeof nodeType === 'undefined' || !nodeType) {
                return;
            }

            const position = reactFlowInstance.screenToFlowPosition({
                x: event.clientX,
                y: event.clientY,
            });

            const newNode: Node = {
                id: `${nodeType.id}_${Date.now()} `,
                type: 'workflowNode',
                position,
                data: {
                    label: nodeType.label,
                    category: nodeType.category,
                    nodeType: nodeType.id,
                    ...nodeType.properties.reduce((acc, prop) => {
                        acc[prop.name] = prop.default;
                        return acc;
                    }, {} as Record<string, any>)
                },
            };

            setNodes((nds: Node[]) => nds.concat(newNode));
        },
        [reactFlowInstance, setNodes],
    );

    const onNodeClick = useCallback((_: React.MouseEvent, node: Node) => {
        onNodeSelect(node);
    }, [onNodeSelect]);

    const onPaneClick = useCallback(() => {
        onNodeSelect(null);
    }, [onNodeSelect]);

    return (
        <div className="canvas-area" ref={reactFlowWrapper}>
            <ReactFlow
                nodes={nodes}
                edges={edges}
                onNodesChange={onNodesChange}
                onEdgesChange={onEdgesChange}
                onConnect={onConnect}
                onInit={setReactFlowInstance}
                onDrop={onDrop}
                onDragOver={onDragOver}
                onNodeClick={onNodeClick}
                onPaneClick={onPaneClick}
                nodeTypes={nodeTypes}
                fitView
            >
                <Controls />
                <MiniMap />
                <Background gap={12} size={1} />
            </ReactFlow>
        </div>
    );
};
