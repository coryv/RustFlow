import React, { useEffect, useState } from 'react';
import type { Node } from 'reactflow';
import { X } from 'lucide-react';

interface PropertiesPanelProps {
    selectedNode: Node | null;
    setNodes: React.Dispatch<React.SetStateAction<Node[]>>;
    setSelectedNode: (node: Node | null) => void;
    nodeTypes: any[];
}

export default function PropertiesPanel({ selectedNode, setNodes, setSelectedNode, nodeTypes }: PropertiesPanelProps) {
    const [data, setData] = useState<any>({});

    useEffect(() => {
        if (selectedNode) {
            setData(selectedNode.data || {});
        }
    }, [selectedNode]);

    const handleChange = (key: string, value: any) => {
        const newData = { ...data, [key]: value };
        setData(newData);

        setNodes((nds) =>
            nds.map((node) => {
                if (node.id === selectedNode?.id) {
                    return { ...node, data: newData };
                }
                return node;
            })
        );
    };

    if (!selectedNode) {
        return (
            <aside className="w-80 bg-gray-900 border-l border-gray-800 p-6 flex flex-col items-center justify-center text-gray-500">
                <p>Select a node to edit properties</p>
            </aside>
        );
    }

    return (
        <aside className="w-80 bg-gray-900 border-l border-gray-800 flex flex-col h-full">
            <div className="p-4 border-b border-gray-800 flex justify-between items-center">
                <div>
                    <h2 className="text-sm font-semibold text-gray-200">Properties</h2>
                    <p className="text-xs text-gray-500">{selectedNode.type}</p>
                </div>
                <button onClick={() => setSelectedNode(null)} className="text-gray-500 hover:text-white">
                    <X size={16} />
                </button>
            </div>

            <div className="p-4 overflow-y-auto flex-1 space-y-4">
                <div>
                    <label className="block text-xs font-medium text-gray-400 mb-1">Label</label>
                    <input
                        type="text"
                        value={data.label || ''}
                        onChange={(e) => handleChange('label', e.target.value)}
                        className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                    />
                </div>

                {/* Dynamic fields based on node definition */}
                {(() => {
                    const nodeType = nodeTypes.find(t => t.id === selectedNode.type);
                    if (!nodeType || !nodeType.properties) return null;

                    return nodeType.properties.map((prop: any) => (
                        <div key={prop.name}>
                            <label className="block text-xs font-medium text-gray-400 mb-1">
                                {prop.label} {prop.required && <span className="text-red-500">*</span>}
                            </label>

                            {prop.type === 'text' && (
                                <input
                                    type="text"
                                    value={data[prop.name] || prop.default || ''}
                                    onChange={(e) => handleChange(prop.name, e.target.value)}
                                    className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                                />
                            )}

                            {prop.type === 'select' && (
                                <select
                                    value={data[prop.name] || prop.default || ''}
                                    onChange={(e) => handleChange(prop.name, e.target.value)}
                                    className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                                >
                                    {prop.options?.map((opt: string) => (
                                        <option key={opt} value={opt}>{opt}</option>
                                    ))}
                                </select>
                            )}

                            {prop.type === 'json' && (
                                <textarea
                                    value={typeof data[prop.name] === 'string' ? data[prop.name] : JSON.stringify(data[prop.name] || JSON.parse(prop.default || "{}"), null, 2)}
                                    onChange={(e) => handleChange(prop.name, e.target.value)}
                                    className="w-full h-40 bg-gray-800 border border-gray-700 rounded px-3 py-2 text-sm text-white font-mono focus:outline-none focus:border-blue-500"
                                />
                            )}

                            {prop.type === 'code' && (
                                <textarea
                                    value={data[prop.name] || prop.default || ''}
                                    onChange={(e) => handleChange(prop.name, e.target.value)}
                                    className="w-full h-60 bg-gray-800 border border-gray-700 rounded px-3 py-2 text-sm text-white font-mono focus:outline-none focus:border-blue-500"
                                />
                            )}
                        </div>
                    ));
                })()}
            </div>
        </aside>
    );
}
