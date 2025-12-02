import React, { useState, useEffect } from 'react';
import { ReactFlowProvider, useNodesState, useEdgesState, type Node } from '@xyflow/react';
import { Sidebar } from './components/Sidebar';
import { FlowCanvas } from './components/FlowCanvas';
import { PropertiesPanel } from './components/PropertiesPanel';
import { fetchNodeTypes } from './api';
import { type NodeType } from './types';
import './layout.css';

function App() {
  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [selectedNode, setSelectedNode] = useState<Node | null>(null);
  const [nodeTypes, setNodeTypes] = useState<NodeType[]>([]);

  useEffect(() => {
    fetchNodeTypes().then(setNodeTypes).catch(console.error);
  }, []);

  const onNodeSelect = (node: Node | null) => {
    setSelectedNode(node);
  };

  const onUpdateNodeData = (id: string, newData: any) => {
    setNodes((nds) =>
      nds.map((node) => {
        if (node.id === id) {
          // Update selected node as well if it's the one being updated
          if (selectedNode && selectedNode.id === id) {
            setSelectedNode({ ...node, data: newData });
          }
          return { ...node, data: newData };
        }
        return node;
      })
    );
  };

  return (
    <div className="app-container">
      <Sidebar />
      <ReactFlowProvider>
        <FlowCanvas
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          setNodes={setNodes}
          setEdges={setEdges}
          onNodeSelect={onNodeSelect}
        />
      </ReactFlowProvider>
      <PropertiesPanel
        selectedNode={selectedNode}
        nodeTypes={nodeTypes}
        onUpdateNodeData={onUpdateNodeData}
      />
    </div>
  );
}

export default App;
