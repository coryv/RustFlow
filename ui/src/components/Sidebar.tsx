import React from 'react';


interface SidebarProps {
    nodeTypes: any[];
}

export default function Sidebar({ nodeTypes }: SidebarProps) {
    const onDragStart = (event: React.DragEvent, nodeType: string, label: string) => {
        event.dataTransfer.setData('application/reactflow', nodeType);
        event.dataTransfer.setData('application/reactflow/label', label);
        event.dataTransfer.effectAllowed = 'move';
    };

    // Group nodes by category
    const categories = nodeTypes.reduce((acc: any, node: any) => {
        const category = node.category || 'Other';
        if (!acc[category]) acc[category] = [];
        acc[category].push(node);
        return acc;
    }, {});

    return (
        <aside className="w-64 bg-gray-900 border-r border-gray-800 flex flex-col h-full">
            <div className="p-4 border-b border-gray-800">
                <h2 className="text-lg font-semibold text-white">Nodes</h2>
                <p className="text-xs text-gray-400 mt-1">Drag nodes to the canvas</p>
            </div>

            <div className="flex-1 overflow-y-auto p-4 space-y-6">
                {Object.entries(categories).map(([category, nodes]: [string, any]) => (
                    <div key={category}>
                        <h3 className="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-3">{category}</h3>
                        <div className="space-y-2">
                            {nodes.map((node: any) => (
                                <div
                                    key={node.id}
                                    className="bg-gray-800 p-3 rounded cursor-move hover:bg-gray-700 transition-colors border border-gray-700 hover:border-blue-500 group"
                                    onDragStart={(event) => onDragStart(event, node.id, node.label)}
                                    draggable
                                >
                                    <div className="flex items-center gap-2 text-sm text-gray-200">
                                        <span>{node.label}</span>
                                    </div>
                                    {node.description && (
                                        <p className="text-xs text-gray-500 mt-1">{node.description}</p>
                                    )}
                                </div>
                            ))}
                        </div>
                    </div>
                ))}
            </div>
        </aside>
    );
}
