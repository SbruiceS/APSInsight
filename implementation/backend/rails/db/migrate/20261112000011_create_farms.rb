# frozen_string_literal: true

class CreateFarms < ActiveRecord::Migration[7.2]
  def change
    create_table :farms do |t|
      t.references :organization, null: false, foreign_key: true
      t.references :project, foreign_key: true
      t.string :name, null: false
      t.string :slug, null: false
      t.text :description
      t.decimal :area_hectares, precision: 12, scale: 4
      t.string :location
      t.timestamps
    end
    add_index :farms, [:organization_id, :slug], unique: true
  end
end
